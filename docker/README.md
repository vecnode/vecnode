# VecNode Docker

Lightweight Alpine Linux container that clones vecnode repository and runs download scripts.

## Reproduce

```bash
docker build -t vecnode:latest .
docker run --rm -it -v $(pwd)/backup:/root/Desktop vecnode:latest

ls
ls ~/Desktop
```


## Cleanup

```bash
docker rm vecnode-backup
docker image rm vecnode:latest
```
