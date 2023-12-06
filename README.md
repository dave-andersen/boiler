# boiler
Monitoring and controlling Dave's boiler via modbus

Currently just a script to log boiler data every minute to json

For the rust code,

    rustup target add arm-unknown-linux-musleabihf
    brew install arm-linux-gnueabihf-binutils

# WARNING

This is a work-in-progress. It probably doesn't even work.
If you use it with your own boiler, you might brick it,
burn down your house, or worse. I'm only leaving this code
public so people can follow along with my little adventure.
It has a lot of hard-coded values specific to my own boiler
& house.
