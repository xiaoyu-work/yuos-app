import QtQuick
import QtQuick.Controls.Basic
import Caelestia.Config
import qs.services

Button {
    id: root

    padding: 8
    hoverEnabled: true
    font.pixelSize: 13

    contentItem: Text {
        text: root.text
        color: root.enabled ? Quiet.ink : Quiet.dim
        textFormat: Text.PlainText
        font: root.font
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }
    background: Rectangle {
        implicitHeight: 32
        radius: Tokens.rounding.small
        color: {
            if (root.enabled) {
                if (root.down)
                    return Quiet.pressed;
                if (root.checked || root.highlighted)
                    return Quiet.selected;
                if (root.hovered)
                    return Quiet.field;
            }
            return Qt.alpha(Quiet.field, 0);
        }
        border.color: root.visualFocus ? Quiet.accent : "transparent"

        Behavior on color {
            ColorAnimation {
                duration: Tokens.anim.durations.expressiveFastEffects
                easing: Tokens.anim.expressiveFastEffects
            }
        }
    }
}
