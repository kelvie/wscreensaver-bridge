# DEPRECATED
I no longer use nor need this, and will use https://wiki.hyprland.org/Hypr-Ecosystem/hypridle/ instead.

# wscreensaver-bridge

This is a D-Bus server that implements
[org.freedesktop.ScreenSaver](https://specifications.freedesktop.org/idle-inhibition-spec/idle-inhibition-spec-latest.html)
and forwards the calls to the [Wayland idle-inhibit
protocol](https://wayland.app/protocols/idle-inhibit-unstable-v1) of your
Wayland compositor.

## Why?

It's a bit of a mess how idleness is inhibited in Linux. There are mechanisms
such as `systemd-inhibit`, and the two mentioned here. The Wayland native way
appears to be through this idle inhibit protocol, which is great, but not all
client programs support this.

Some of them use this, and rather than track down all the programs that do and
ask them to rewrite their screensaver inhibitors, I figure there may as well be
a bridge/adapter.

## Install/build

On Arch, just install it from the AUR, e.g. using
[yay](https://github.com/Jguer/yay):

 ``` sh
    yay -S wscreensaver-bridge-git
 ```

 To build, just install rust and make sure you have the dbus development
 packages.

## Use

Just run it when your session starts, e.g. in `hyprland.conf`:

```
exec-once wscreensaver-bridge
```

Or write a systemd user unit for it:

`~/.config/systemd/user/wscreensaver-bridge.service`:
```
[Unit]
Description=Wayland Screensaver Bridge

[Service]
ExecStart=/usr/bin/wscreensaver-bridge
Restart=always

[Install]
WantedBy=default.target
```

Then enable it with `systemctl --user enable wscreensaver-bridge.service`.
