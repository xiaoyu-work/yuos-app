pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Shapes
import Quickshell
import Quickshell.Bluetooth
import Quickshell.Wayland
import Quickshell.Services.UPower
import Caelestia.I18n
import qs.components
import qs.components.containers
import qs.services

StyledWindow {
    id: root

    signal menuRequested(string name)
    signal pinMenuRequested(string desktopId, real position)
    signal launched()

    property string menuPage
    property Item hoveredApp
    readonly property string dockTooltip: {
        if (!hoveredApp || hoverDelay.running || dock.moving)
            return "";
        const { name, description } = hoveredApp.Accessible;
        return description ? `${name} \u00b7 ${description}` : name;
    }
    readonly property real dockTooltipX: hoveredApp
        ? leading.x + dock.x + hoveredApp.x - dock.contentX + hoveredApp.width / 2 : 0

    onHoveredAppChanged: hoveredApp ? hoverDelay.restart() : hoverDelay.stop()

    Timer {
        id: hoverDelay
        interval: 450
    }

    name: "yuos-top-bar"
    anchors {
        top: true
        left: true
        right: true
    }
    implicitHeight: 44
    exclusiveZone: implicitHeight
    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None

    QuietSurface {
        anchors.fill: parent
        radius: 0
        border.width: 0

        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: 1
            color: Quiet.line
        }
    }

    Row {
        id: leading

        anchors.left: parent.left
        anchors.leftMargin: 8
        anchors.verticalCenter: parent.verticalCenter
        height: 36
        spacing: 5

        QuietButton {
            width: 36
            height: 36
            objectName: "taskbarLogo"
            Accessible.name: Tr.tr("Open Yu AI")
            ToolTip.visible: hovered
            ToolTip.delay: 450
            ToolTip.text: Accessible.name
            onClicked: {
                root.launched();
                DesktopMenu.launch(DesktopMenu.agent);
            }

            contentItem: Item {
                Shape {
                    anchors.centerIn: parent
                    width: 26
                    height: 26
                    preferredRendererType: Shape.CurveRenderer

                    ShapePath {
                        strokeColor: Quiet.ink
                        strokeWidth: 2.8
                        capStyle: ShapePath.RoundCap
                        fillColor: "transparent"
                        PathSvg { path: "M12.5 3.8 3.1 21.7 M15.9 10.2 9.7 21.7 M20 14.4 16.1 21.7" }
                    }
                }
                Rectangle {
                    x: (parent.width - 26) / 2 + 21.2
                    y: (parent.height - 26) / 2 + 20.4
                    width: 3.2
                    height: width
                    radius: width / 2
                    color: Quiet.ink
                }
            }
        }

        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            width: 1
            height: 24
            color: Quiet.line
        }

        ListView {
            id: dock

            width: Math.min(Math.max(0, count * 38 - 3),
                Math.max(0, status.x - leading.x - 56))
            height: 36
            orientation: ListView.Horizontal
            boundsBehavior: Flickable.StopAtBounds
            spacing: 3
            clip: true
            reuseItems: true
            model: Quiet.dock

            WheelHandler {
                orientation: Qt.Vertical
                onWheel: event => dock.contentX = Math.max(0,
                    Math.min(dock.contentWidth - dock.width, dock.contentX - event.angleDelta.y))
            }

            delegate: QuietButton {
                id: app

                required property string desktopId
                required property bool appRunning
                required property string focusedOutput
                readonly property bool waiting: DesktopMenu.pendingDesktopId === desktopId || Quiet.pendingDockId === desktopId
                readonly property real iconSize: 25
                readonly property real hoverScale: 1.08
                // Qt applies the output's DPR to image-provider requests. Budget only for peak hover.
                readonly property int iconLoadSize: Math.ceil(iconSize * hoverScale)

                width: 35
                height: 36
                padding: 3
                highlighted: focusedOutput !== "" && focusedOutput === root.screen?.name
                Accessible.name: Quiet.title(desktopId)
                Accessible.description: waiting ? Tr.tr("Waiting for Yu OS") : ""
                onHoveredChanged: {
                    if (hovered)
                        root.hoveredApp = app;
                    else
                        clearHover();
                }
                ListView.onPooled: clearHover()

                function clearHover(): void {
                    if (root.hoveredApp === app)
                        root.hoveredApp = null;
                }

                onClicked: {
                    clearHover();
                    root.launched();
                    Quiet.activate(desktopId);
                }

                contentItem: Item {
                    Image {
                        anchors.horizontalCenter: parent.horizontalCenter
                        anchors.top: parent.top
                        anchors.topMargin: 1
                        width: app.iconSize
                        height: app.iconSize
                        sourceSize: Qt.size(app.iconLoadSize, app.iconLoadSize)
                        source: Quiet.icon(app.desktopId)
                        fillMode: Image.PreserveAspectFit
                        asynchronous: true
                        scale: app.down ? 0.94 : app.hovered && !dock.moving ? app.hoverScale : 1

                        Behavior on scale {
                            Anim { type: Anim.FastEffects }
                        }
                    }

                    Rectangle {
                        anchors.horizontalCenter: parent.horizontalCenter
                        anchors.bottom: parent.bottom
                        width: app.highlighted ? 10 : app.waiting ? 8 : 4
                        height: 3
                        radius: height / 2
                        color: app.highlighted ? Quiet.accent : app.waiting ? "transparent" : Quiet.dim
                        border.color: Quiet.accent
                        border.width: app.waiting && !app.highlighted ? 1 : 0
                        visible: app.appRunning || app.waiting

                        Behavior on width {
                            Anim { type: Anim.FastEffects }
                        }
                    }
                }

                TapHandler {
                    acceptedButtons: Qt.RightButton
                    onTapped: {
                        app.clearHover();
                        if (app.desktopId !== DesktopMenu.launcher)
                            root.pinMenuRequested(app.desktopId, app.mapToItem(leading, 0, 0).x + leading.x);
                    }
                }
            }
        }

    }

    Row {
        id: status

        anchors.right: parent.right
        anchors.rightMargin: 8
        anchors.verticalCenter: parent.verticalCenter
        height: 36
        spacing: 2

        QuietButton {
            width: 60
            height: 36
            text: Time.format("hh:mm")
            font.weight: Font.DemiBold
            highlighted: root.menuPage === "overview"
            Accessible.name: Tr.tr("Desktop overview")
            onClicked: root.menuRequested("overview")
        }

        QuietButton {
            width: UPower.displayDevice.isLaptopBattery ? 108 : 56
            height: 36
            highlighted: root.menuPage === "control" || root.menuPage === "notifications"
            Accessible.name: Tr.tr("Control Center")
            ToolTip.text: Accessible.name
            ToolTip.visible: hovered
            ToolTip.delay: 450
            onClicked: root.menuRequested("control")

            contentItem: Item {
                Row {
                    anchors.centerIn: parent
                    spacing: 6
                    StatusIcon {
                        drawing: Nmcli.activeEthernet
                            ? "M4 3 H12 V9 H4 Z M8 9 V13 M4 13 H12"
                            : "M1.5 5 Q8 -0.5 14.5 5 M4 8 Q8 4.5 12 8 M6.5 11 Q8 9.5 9.5 11 M8 13.5 H8.1"
                        opacity: Nmcli.activeEthernet || Nmcli.active ? 1 : 0.6
                    }
                    StatusIcon {
                        drawing: "M8 1.5 L12 5.2 4 11.6 M8 14.5 L12 10.8 4 4.4 M8 1.5 V14.5"
                        opacity: Bluetooth.defaultAdapter?.enabled ? 1 : 0.6
                    }
                    StatusIcon {
                        drawing: "M1.5 4.5 H13 V11.5 H1.5 Z M14.5 6 V10"
                        visible: UPower.displayDevice.isLaptopBattery
                    }
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: UPower.displayDevice.isLaptopBattery
                            ? `${Math.round(UPower.displayDevice.percentage * 100)}%` : ""
                        visible: text.length > 0
                        color: Quiet.accent
                        font.pixelSize: 12
                    }
                }
            }
        }

        QuietButton {
            width: 36
            height: 36
            highlighted: root.menuPage === "power" || root.menuPage === "logout"
            Accessible.name: Tr.tr("Power menu")
            ToolTip.visible: hovered
            ToolTip.text: Accessible.name
            ToolTip.delay: 450
            contentItem: Item {
                StatusIcon {
                    anchors.centerIn: parent
                    drawing: "M8 1.5 V7 M4.3 4.2 A5.4 5.4 0 1 0 11.7 4.2"
                }
            }
            onClicked: root.menuRequested("power")
        }
    }

    component StatusIcon: Shape {
        id: icon

        required property string drawing

        width: 16
        height: 16
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeColor: Quiet.accent
            strokeWidth: 1.4
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            fillColor: "transparent"
            PathSvg { path: icon.drawing }
        }
    }
}
