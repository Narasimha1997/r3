#!/bin/bash

# this is the name of the physical interface you are using on the host
# run ifconfig for more info
MY_IFACE="enp0s25"

brctl addbr brr3
ip addr flush dev $MY_IFACE
brctl addif brr3 $MY_IFACE
tunctl -t tapr3 -u `whoami`
brctl addif brr3 tapr3

ifconfig $MY_IFACE up
ifconfig tapr3 up
ifconfig brr3 up

dhclient -v brr3