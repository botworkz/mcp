VERSION 0.8

mcp-echo-image:
    ARG BINARY_SOURCE=source
    ARG TAG=latest
    FROM DOCKERFILE --platform=linux/amd64 --build-arg BINARY_SOURCE=$BINARY_SOURCE -f containers/mcp-echo/Dockerfile .
    SAVE IMAGE botwork/mcp-echo:local
    IF [ "$TAG" = "latest" ]
        SAVE IMAGE --push ghcr.io/botworkz/mcp/mcp-echo:latest
    ELSE
        SAVE IMAGE --push ghcr.io/botworkz/mcp/mcp-echo:${TAG}
        SAVE IMAGE --push ghcr.io/botworkz/mcp/mcp-echo:latest
    END

images:
    BUILD +mcp-echo-image
