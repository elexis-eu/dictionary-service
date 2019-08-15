FROM debian:stretch-slim 

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
        sqlite3

ADD target/release/elexis-dictionary-service /bin/

ENTRYPOINT ["/bin/elexis-dictionary-service"]
