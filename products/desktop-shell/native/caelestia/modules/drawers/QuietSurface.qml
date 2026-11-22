import QtQuick
import Caelestia.Config
import qs.services

Rectangle {
    radius: Tokens.rounding.medium
    border.color: Quiet.line
    gradient: Gradient {
        GradientStop { position: 0; color: Quiet.panelTop }
        GradientStop { position: 1; color: Quiet.panel }
    }
}
