#!/usr/bin/env bash

podman build -t umq .
podman image scp $USER@localhost::umq root@localhost:: > /dev/null
