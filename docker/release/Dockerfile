FROM debian as installer
ARG version
COPY ./scripts/installer/install-databend.sh /install.sh
RUN apt update \
    && apt install curl tar -y
# hack ssl issue in curl
RUN update-ca-certificates -f
RUN chmod +x /install.sh
RUN /install.sh $version
# double check whether binaries could run on host architecture
RUN timeout 3 /root/.databend/bin/databend-query || true
RUN timeout 3 /root/.databend/bin/databend-dfs || true
# CI environment build image under GLIBC 2.29 and debian:buster only run GLIBC 2.28
FROM debian:bullseye
COPY --from=installer /root/.databend/bin/databend-query  /databend-query
COPY --from=installer /root/.databend/bin/databend-dfs /databend-dfs
COPY ./docker/bootstrap.sh /bootstrap.sh
COPY ./docker/databend-query-docker.toml  /databend-query.toml
ENTRYPOINT ["/bootstrap.sh"]