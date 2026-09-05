FROM alpine:3.19 AS rust-builder
WORKDIR /app
RUN echo "rust-builder stage" > /app/build.txt

FROM alpine:3.19 AS go-builder
WORKDIR /app
RUN echo "go-builder stage" > /app/build.txt

FROM alpine:3.19 AS ts-builder
WORKDIR /app
RUN echo "ts-builder stage" > /app/build.txt

FROM alpine:3.19 AS ubuntu-22-04
WORKDIR /app
COPY --from=rust-builder /app/build.txt /usr/local/bin/rust-service
COPY --from=go-builder /app/build.txt /usr/local/bin/go-service
EXPOSE 8080 8081
CMD ["rust-service"]
