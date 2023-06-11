# X410 WSL2 Helper
# https://x410.dev/cookbook/#wsl
# --------------------
# Setting up D-Bus for WSL Ubuntu
# --------------------
#
# D-Bus is used by kcachegrind
#
# '/run/user' directory is always empty when WSL2 is first
# launched; a perfect time to setup daemons and D-Bus
#

export XDG_RUNTIME_DIR=/run/user/$(id -u)
if [ ! -d "$XDG_RUNTIME_DIR" ]; then
{
    # Create user runtime directories
    sudo mkdir $XDG_RUNTIME_DIR && \
    sudo chmod 700 $XDG_RUNTIME_DIR && \
    sudo chown $(id -un):$(id -gn) $XDG_RUNTIME_DIR

    # System D-Bus
    sudo service dbus start

    # --------------------
    # Start additional services as they are needed.
    # We recommend adding commands that require 'sudo' here. For other
    # regular background processes, you should add them below where a
    # session 'dbus-daemon' is started.
    # --------------------
}
fi

set_session_dbus()
{
    local bus_file_path="$XDG_RUNTIME_DIR/bus"

    export DBUS_SESSION_BUS_ADDRESS=unix:path=$bus_file_path
    if [ ! -e "$bus_file_path" ]; then
    {
        /usr/bin/dbus-daemon --session --address=$DBUS_SESSION_BUS_ADDRESS --nofork --nopidfile --syslog-only &

        # --------------------
        # More background processes can be added here.
        # For 'sudo' requiring commands, you should add them above
        # where the 'dbus' service is started.
        # --------------------

    }
    fi
}

set_session_dbus
