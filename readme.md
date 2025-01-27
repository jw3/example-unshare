example unsharing ipc namespaces
===

Demonstrates the `unshare`ing of a processes into a new ipc namespace

Use podman to access the ipc namespace while demonstrating the namespace isolation.

### tools 

1. umq - helper to list / and send to the mq
2. Podman - namespace flexible containers 

### Example commands used in testing

1. List all Qs `#! sudo umq ls`
2. Make a new Q `#! sudo umq mk /foo`
3. Unshare a new Q `#! sudo umq mk -u /bar`
4. Unshare a Q with system util `#! sudo unshare umq mk /baz`
5. Unshare q Q into new ipc namespace `#! sudo unshare -i umq mk /biz`
6. Run podman container, attching to ipc ns `#! sudo podman run --rm -it --ipc=ns:/proc/498540/ns/ipc umq bash`
7. Enter an existing ns by pid `sudo nsenter -t 498540 --ipc=/proc/498540/ns/ipc --mount=/proc/498540/ns/mnt --all  bash`

## Demonstration

The demonstration uses Podman and the `umq` tool from this repo

```text
Usage: umq [OPTIONS] <COMMAND>

Commands:
  rm    Remove one or more message queues
  ls    List all message queues from the current namespace
  mk    Make a new message queue, with options
  rx    Receive messages from message queue, with options
  tx    Send message to a message queue, with options
  help  Print this message or the help of the given subcommand(s)

Options:
      --image-name <IMAGE_NAME>  [default: example]
  -h, --help                     Print help
```

See `--help` for each command for more info.

### Prep
1. Build example container image `sudo ./podman-build.sh`
2. Build or download the `umq` executable 

On Ubuntu 24.04 some commands require App Armor config
- `sudo sysctl kernel.apparmor_restrict_unprivileged_userns=0`

### Steps
1. Demonstrate an unshared namespace and the visibility of it.
2. Demonstrate creating a message queue without unsharing the namespace.
3. Demonstrate connecting a container to the namespace

### Commmands
1. List, zero mq
    `sudo ./umq ls`
2. Make mq /foo
    `sudo ./umq mk /foo`
3. List, one mq
   `sudo ./umq ls`
4. Make unshared mq /bar and wait for a message
   `sudo ./umq mk -w -u /bar &`
5. List, still only one mq, /foo
   `sudo ./umq ls`
6. Start a container, attach to the namespace created in 4
   `sudo podman run --rm -it --ipc=ns:/proc/366371/ns/ipc -v $HOME/.cargo/bin:/usr/local/bin:ro ubuntu:22.04 bash`
7. List, one mq, /bar
8. Send a message to /bar and observe it received
9. 

## Examples

### Create queue /fizz in host namespace
```
umq mk /fizz
```

### Send message to /fizz
```
umq tx /fizz hello
```

### Find fizz in container
```
podman run --rm -it umq ls
```
Not found
```
sudo podman run --rm -it umq ls
```
Not found
```
podman run --rm -it --ipc=host umq ls
```
Found, in host namespace

### Send message from Podman to /fizz in host namespace
```
podman run --rm -it --ipc=host umq tx /fizz 'Hello, from podman'
```

### Create queue in new namespace

```
sudo umq mk -v -u /foo
```

### View and Send message to namespaces queue

View namespace
```
sudo podman run --rm -it umq ls
```
Not found, wrong namespace

```
sudo podman run --rm -it --ipc=ns:/proc/337733/ns/ipc umq ls
/foo: QSIZE:0          NOTIFY:0     SIGNO:0     NOTIFY_PID:0     
```

Found in specified namespace

Send message

```
sudo podman run --rm -it --ipc=ns:/proc/337733/ns/ipc umq tx /foo 'Hello, from podman'
```

Received

## Using Podman shared namespace

```
podman run --rm -it --ipc=shareable --name=ipc-master umq mk -v /bar
```

```text
sudo podman run --rm -it umq ls
```

No queues found

Cannot map directly to pid of ns because it is pid 1 in the container

Use podman container ipc instead

```text
podman run --rm -it --ipc=container:ipc-master umq ls
```

Queue found, send message

```text
podman run --rm -it --ipc=container:ipc-master umq tx /bar hello
```

Message recieved
