#!/usr/bin/env sh

set -xe
docker build -t xrlx/graphite_build .
# run the build env CMD for copying deb and installing to minimal image
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock -ti xrlx/graphite_build

