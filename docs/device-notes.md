# Device notes

The Kindle driver refreshes the screen's active idle timer while a visible focus or break session is running.
Its local tests cover the four-minute cadence, pause, and manual sleep. The driver checks the device power state
before sending the refresh and leaves the screensaver setting alone. [KOReader's Kindle power integration](https://github.com/koreader/koreader/blob/master/frontend/device/kindle/powerd.lua#L1421-L1442)

On the Oasis 3, verify that a timer stays visible beyond the stock battery-powered idle timeout. Press the power
button during the session, wake the device, and confirm the timer still reflects elapsed time. Also verify pause,
exit, charging, rotation, and completion across sleep before a device release.
