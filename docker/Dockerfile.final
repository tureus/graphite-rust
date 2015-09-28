FROM busybox

ADD carbon /usr/bin/carbon

ENV RUST_LOG debug

VOLUME /data
EXPOSE 2003
EXPOSE 2003/udp

ENTRYPOINT ["/usr/bin/carbon", "--storage-path", "/data"]
