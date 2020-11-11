FROM golang:alpine AS build-env

# Set up dependencies
ENV PACKAGES git build-base

# Set working directory for the build
WORKDIR /go/src/github.com/aragon/aragon-chain

# Install dependencies
RUN apk add --update $PACKAGES
RUN apk add linux-headers

# Add source files
COPY . .

# Make the binary
RUN make build

# Final image
FROM alpine

# Install ca-certificates
RUN apk add --update ca-certificates
WORKDIR /root

# Copy over binaries from the build-env 
COPY --from=build-env /go/src/github.com/aragon/aragon-chain/build/aragonchaind /usr/bin/aragonchaind
COPY --from=build-env /go/src/github.com/aragon/aragon-chain/build/aragonchaincli /usr/bin/aragonchaincli

EXPOSE 26656 26657 1317 8545 8546

# Run aragonchaind by default
ENTRYPOINT ["/bin/bash"]