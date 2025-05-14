import SwiftUI
import AppKit
import UniformTypeIdentifiers

struct FilePicker: NSViewControllerRepresentable {
    let onPick: (URL?) -> Void

    func makeCoordinator() -> Coordinator {
        return Coordinator(onPick: onPick)
    }

    func makeNSViewController(context: Context) -> NSViewController {
        let controller = NSViewController()
        DispatchQueue.main.async {
            let panel = NSOpenPanel()
            panel.title = "Choose a .mpb Bundle"
            panel.canChooseDirectories = true
            panel.canChooseFiles = false
            if #available(macOS 11.0, *) {
                panel.allowedContentTypes = [UTType(filenameExtension: "mpb")!]
            } else {
                panel.allowedFileTypes = ["mpb"]
            }
            panel.allowsMultipleSelection = false

            if panel.runModal() == .OK {
                context.coordinator.onPick(panel.url)
            } else {
                context.coordinator.onPick(nil)
            }
        }
        return controller
    }

    func updateNSViewController(_ nsViewController: NSViewController, context: Context) {}

    class Coordinator {
        let onPick: (URL?) -> Void
        init(onPick: @escaping (URL?) -> Void) {
            self.onPick = onPick
        }
    }
}

struct ContentView: View {
    @State private var bundlePath: String = ""
    @State private var outputMessage: String = ""
    @State private var selectedPath: String?
    @State private var showingPicker: Bool = false
    
    var body: some View {
        VStack {
            TextField("Path to bundle (.mpb)...", text: Binding(
                get: { selectedPath ?? "" },
                set: { selectedPath = $0 }
            ))
            .padding()
            .border(Color.gray, width: 1)
            
            Button("Select .mpb Bundle") {
                showingPicker = true
            }
            
            Button(action: {
                runBundle(bundlePath: bundlePath)
            }) {
                Text("Run")
                    .padding()
                    .cornerRadius(5)
            }
            .padding()
            
            Text(outputMessage)
                .padding()
            
        }
        .padding()
        .sheet(isPresented: $showingPicker) {
            FilePicker { url in
                if let url = url {
                    selectedPath = url.path
                }
                showingPicker = false
                bundlePath = selectedPath.unsafelyUnwrapped
            }
        }
    }
    
    func runBundle(bundlePath: String) {
        let process = Process()
        let pipe = Pipe()
        
        // Resolve absolute path for bundle
        let homeDir = FileManager.default.homeDirectoryForCurrentUser
        let macpackPath = homeDir.appendingPathComponent(".macpack/bin/macpack")
        
        // Resolve the full absolute path for the bundlePath if it's not absolute
        let fullBundlePath = URL(fileURLWithPath: bundlePath, isDirectory: true).standardized
        
        process.executableURL = macpackPath
        process.arguments = [fullBundlePath.path]  // Pass the absolute bundle path as the argument
        process.standardOutput = pipe
        process.standardError = pipe
        
        do {
            try process.run()
            process.waitUntilExit()
            
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            if let output = String(data: data, encoding: .utf8) {
                outputMessage = output
            }
        } catch {
            outputMessage = "Error running Rust command: \(error)"
            let pasteboard = NSPasteboard.general
            pasteboard.declareTypes([.string], owner: nil)
            pasteboard.setString("\(error)", forType: .string)
        }
    }
}
