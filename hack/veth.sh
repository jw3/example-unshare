#!/usr/bin/env bash

up() {
  ip link add veth0 type veth peer name veth1
  ip link set veth0 up
  ip link set veth1 up
  ip addr add 192.168.100.1/24 dev veth0
  ip addr add 192.168.100.2/24 dev veth1
  echo "ready: veth0 @ 192.168.100.{1,2}"
}

down() {
  ip link set macvtap0 down
  ip link delete macvtap0
  ip link set br0 down
  ip link delete br0
  ip link set veth0 down
  ip link set veth1 down
  ip link delete veth0
}

bridge() {
  ip link add veth0 type veth peer name veth1
  ip link add name br0 type bridge
  ip link set veth0 master br0
  ip link set veth0 up
  ip link set veth1 up
  ip link set br0 up
  ip addr add 192.168.100.1/24 dev br0
  ip addr add 192.168.100.2/24 dev veth1
  echo "ready: br0, veth1 @ 192.168.100.{1,2}"
}

vtap() {
    ip link set veth1 up
    ip link add link veth1 name macvtap0 type macvtap mode bridge
    ip link set macvtap0 up

    IFINDEX=$(cat /sys/class/net/macvtap0/ifindex)
    TAP_DEV="/dev/tap${IFINDEX}"
    sudo chown qemu:qemu "$TAP_DEV"
    echo "ready: attach $TAP_DEV to vm"
}

"$@"
