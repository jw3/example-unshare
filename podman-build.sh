#!/usr/bin/env bash

podman build -t example .
podman image scp $USER@localhost::example root@localhost:: > /dev/null
