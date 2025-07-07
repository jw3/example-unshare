#!/bin/bash
set -euo pipefail

pid=$(sudo -u rocky podman inspect --format '{{.State.Pid}}' mynet)

ns="/proc/$pid/ns/net"
if [ ! -e "$ns" ]; then
    echo "error: netns for container not found at $ns"
    exit 1
fi

mkdir -p /run/netns
ln -sfT "$ns" /run/netns/mynetcontainer

ip link set enp9s0 down
ip link set enp9s0 netns mynetcontainer
ip netns exec mynetcontainer ip link set enp9s0 up
ip netns exec mynetcontainer ip addr add 192.168.100.3/24 dev enp9s0
