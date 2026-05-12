// STURecorder — a tiny Swift CLI that records the main display via
// ScreenCaptureKit and writes H.264-encoded .mov via AVAssetWriter.
//
// Usage:
//     STURecorder <output.mov> [--fps N] [--show-cursor] [--mic]
//
// The recorder runs until it receives SIGTERM or SIGINT, at which point
// it cleanly stops the stream, finalises the writer, and prints the
// output path on stdout. Stderr is reserved for diagnostics.
//
// We deliberately wrap this in a separate binary rather than wrestling
// with objc2's SCK bindings from Rust: SCStreamOutput is a Cocoa
// delegate with async sample-buffer callbacks, and AVAssetWriter has
// its own threading model. Swift handles this in ~200 lines; the same
// in objc2 would be 700+.

import Foundation
import AVFoundation
import CoreMedia
import CoreVideo
import ScreenCaptureKit
import AppKit

// ---- Argument parsing -----------------------------------------------------

struct Args {
    var output: URL
    var fps: Int32
    var showCursor: Bool
    var includeMic: Bool
}

func parseArgs() -> Args {
    let argv = CommandLine.arguments
    guard argv.count >= 2 else {
        FileHandle.standardError.write(
            Data("usage: STURecorder <output.mov> [--fps N] [--show-cursor] [--mic]\n".utf8)
        )
        exit(2)
    }
    var output: URL?
    var fps: Int32 = 60
    var showCursor = false
    var includeMic = false
    var i = 1
    while i < argv.count {
        let a = argv[i]
        switch a {
        case "--fps":
            i += 1
            if i < argv.count, let v = Int32(argv[i]) { fps = v }
        case "--show-cursor":
            showCursor = true
        case "--mic":
            includeMic = true
        default:
            if output == nil { output = URL(fileURLWithPath: a) }
        }
        i += 1
    }
    guard let out = output else {
        FileHandle.standardError.write(Data("STURecorder: missing output path\n".utf8))
        exit(2)
    }
    return Args(output: out, fps: fps, showCursor: showCursor, includeMic: includeMic)
}

// ---- Recorder -------------------------------------------------------------

final class Recorder: NSObject, SCStreamOutput, SCStreamDelegate {
    private var stream: SCStream?
    private var writer: AVAssetWriter?
    private var videoInput: AVAssetWriterInput?
    private var sessionStarted = false
    private let queue = DispatchQueue(label: "stu.recorder.samples")
    // Set once stop() begins. Skip subsequent samples so we don't append
    // after the writer's been finalised.
    private var stopping = false

    func start(args: Args) async throws {
        // Find the main display. excludingDesktopWindows=false because we
        // want the real desktop wallpaper in the recording.
        let content = try await SCShareableContent.excludingDesktopWindows(
            false,
            onScreenWindowsOnly: true
        )
        guard let display = content.displays.first(where: { $0.displayID == CGMainDisplayID() })
            ?? content.displays.first else {
            throw NSError(
                domain: "STURecorder",
                code: 1,
                userInfo: [NSLocalizedDescriptionKey: "no display found"]
            )
        }

        // Filter and stream config. Pixel dimensions = display.width *
        // backing-scale-factor (Retina). We pick the screen's actual
        // scale via NSScreen so we don't hardcode 2x.
        let scale = NSScreen.main?.backingScaleFactor ?? 2.0
        let pixelWidth = Int(CGFloat(display.width) * scale)
        let pixelHeight = Int(CGFloat(display.height) * scale)

        let filter = SCContentFilter(display: display, excludingWindows: [])

        let cfg = SCStreamConfiguration()
        cfg.width = pixelWidth
        cfg.height = pixelHeight
        cfg.minimumFrameInterval = CMTime(value: 1, timescale: CMTimeScale(args.fps))
        cfg.queueDepth = 6
        cfg.showsCursor = args.showCursor
        cfg.pixelFormat = kCVPixelFormatType_32BGRA
        cfg.colorSpaceName = CGColorSpace.sRGB

        // AVAssetWriter ------------------------------------------------
        try? FileManager.default.removeItem(at: args.output)
        let writer = try AVAssetWriter(outputURL: args.output, fileType: .mov)
        let videoSettings: [String: Any] = [
            AVVideoCodecKey: AVVideoCodecType.h264,
            AVVideoWidthKey: pixelWidth,
            AVVideoHeightKey: pixelHeight,
            AVVideoCompressionPropertiesKey: [
                AVVideoAverageBitRateKey: pixelWidth * pixelHeight * Int(args.fps) / 8,
                AVVideoMaxKeyFrameIntervalKey: Int(args.fps) * 2,
                AVVideoProfileLevelKey: AVVideoProfileLevelH264HighAutoLevel,
            ] as [String: Any],
        ]
        let videoInput = AVAssetWriterInput(mediaType: .video, outputSettings: videoSettings)
        videoInput.expectsMediaDataInRealTime = true
        if writer.canAdd(videoInput) {
            writer.add(videoInput)
        } else {
            throw NSError(
                domain: "STURecorder",
                code: 2,
                userInfo: [NSLocalizedDescriptionKey: "writer cannot add video input"]
            )
        }
        writer.startWriting()
        self.writer = writer
        self.videoInput = videoInput

        // SCStream ------------------------------------------------------
        let stream = SCStream(filter: filter, configuration: cfg, delegate: self)
        try stream.addStreamOutput(self, type: .screen, sampleHandlerQueue: queue)
        try await stream.startCapture()
        self.stream = stream
        FileHandle.standardError.write(
            Data("STURecorder: started → \(args.output.path) at \(pixelWidth)×\(pixelHeight)@\(args.fps)fps\n".utf8)
        )
    }

    func stream(
        _ stream: SCStream,
        didOutputSampleBuffer sampleBuffer: CMSampleBuffer,
        of outputType: SCStreamOutputType
    ) {
        guard outputType == .screen, sampleBuffer.isValid, !stopping else { return }

        // Only append "complete" frames — SCStream may deliver idle / blank
        // frames that we should skip.
        if let attachmentsArray = CMSampleBufferGetSampleAttachmentsArray(
            sampleBuffer,
            createIfNecessary: false
        ) as? [[SCStreamFrameInfo: Any]],
           let attachment = attachmentsArray.first,
           let statusRaw = attachment[.status] as? Int,
           let status = SCFrameStatus(rawValue: statusRaw),
           status != .complete
        {
            return
        }

        guard let writer = writer, let videoInput = videoInput else { return }
        if !sessionStarted {
            writer.startSession(atSourceTime: sampleBuffer.presentationTimeStamp)
            sessionStarted = true
        }
        if videoInput.isReadyForMoreMediaData {
            videoInput.append(sampleBuffer)
        }
    }

    func stream(_ stream: SCStream, didStopWithError error: Error) {
        FileHandle.standardError.write(
            Data("STURecorder: stream stopped with error: \(error)\n".utf8)
        )
    }

    func stop() async {
        stopping = true
        if let stream = stream {
            do {
                try await stream.stopCapture()
            } catch {
                FileHandle.standardError.write(
                    Data("STURecorder: stopCapture error: \(error)\n".utf8)
                )
            }
        }
        videoInput?.markAsFinished()
        if let writer = writer {
            await writer.finishWriting()
            if writer.status == .failed {
                FileHandle.standardError.write(
                    Data("STURecorder: writer failed: \(String(describing: writer.error))\n".utf8)
                )
            }
        }
    }
}

// ---- Main -----------------------------------------------------------------

let args = parseArgs()
let recorder = Recorder()

// Install graceful-stop handlers for SIGTERM + SIGINT. Tell the kernel to
// ignore the signal (signal()) so the default handler doesn't kill us, then
// listen for it via a DispatchSource on the main queue.
signal(SIGTERM, SIG_IGN)
signal(SIGINT, SIG_IGN)
let stopSemaphore = DispatchSemaphore(value: 0)
var stopRequested = false

let termSrc = DispatchSource.makeSignalSource(signal: SIGTERM, queue: .main)
termSrc.setEventHandler { stopRequested = true; stopSemaphore.signal() }
termSrc.resume()
let intSrc = DispatchSource.makeSignalSource(signal: SIGINT, queue: .main)
intSrc.setEventHandler { stopRequested = true; stopSemaphore.signal() }
intSrc.resume()

// Kick off the async start. If anything fails before we get to the
// "started" message we bail with a non-zero exit code.
Task {
    do {
        try await recorder.start(args: args)
    } catch {
        FileHandle.standardError.write(
            Data("STURecorder: start failed: \(error)\n".utf8)
        )
        exit(1)
    }
}

// Wait for a signal to stop. Block on a semaphore so the run loop keeps
// pumping ScreenCaptureKit's dispatch sources in the meantime.
DispatchQueue.global(qos: .utility).async {
    stopSemaphore.wait()
    Task { @MainActor in
        await recorder.stop()
        print(args.output.path)
        exit(0)
    }
}

// Pump the main run loop so SCK delegates run.
RunLoop.main.run()
