FROM rust:alpine AS build

COPY . /app

WORKDIR /app

RUN apk add --no-cache gcc g++ zlib zlib-dev

RUN cargo build --release

FROM alpine as runtime

COPY --from=build app/target/release/transmission_api_client /

CMD [ "./transmission_api_client" ]
 
