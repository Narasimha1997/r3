#!/bin/bash

MY_IFACE="wlp13s0"

brctl delif brr3 tapr3
tunctl -d tapr3
brctl delif brr3 $MY_IFACE
ifconfig brr3 down
brctl delbr brr3
ifconfig $MY_IFACE up
dhclient -v $MY_IFACE
