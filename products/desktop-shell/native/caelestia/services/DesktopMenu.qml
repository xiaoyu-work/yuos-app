pragma Singleton
pragma ComponentBehavior: Bound

import Quickshell
import Quickshell.Io
import Caelestia
import Caelestia.I18n

Singleton {
    id: root

    readonly property string files: "com.yuos.Files"
    readonly property string launcher: "com.yuos.Launcher"
    readonly property string agent: "com.yuos.Agent"
    readonly property alias pendingDesktopId: effect.desktopId

    function launch(desktopId: string): void {
        submit("launch", desktopId);
    }

    function activate(desktopId: string): void {
        submit("activate", desktopId);
    }

    function submit(action: string, desktopId: string): void {
        if (effect.running || pendingDesktopId) {
            if (pendingDesktopId !== desktopId)
                Toaster.toast(Tr.tr("Application launch busy"), Tr.tr("Wait for the current launch request to finish."), "error");
            return;
        }
        effect.desktopId = desktopId;
        effect.command = ["/usr/bin/python3", Quickshell.shellPath("utils/scripts/desktop_review.py"), action, desktopId];
        effect.running = true;
    }

    function finishLaunch(error: string): void {
        effect.desktopId = "";
        if (error)
            Toaster.toast(Tr.tr("Application did not open"), error, "error");
    }

    Process {
        id: effect

        property string desktopId
        stdout: StdioCollector {
            id: result
        }

        onExited: exitCode => {
            let reply = null;
            if (result.text.trim() || exitCode === 0) {
                try {
                    reply = JSON.parse(result.text);
                } catch (error) {
                    console.warn("Invalid desktop launch response:", error);
                }
            }
            const error = typeof reply?.error === "string" && reply.error
                ? reply.error : Tr.tr("Yu OS did not confirm the launch request.");
            root.finishLaunch(exitCode === 0 && reply?.submitted === true ? "" : error);
        }
        onRunningChanged: {
            if (desktopId && !running)
                root.finishLaunch(Tr.tr("The OS client could not be started."));
        }
    }
}
