VERSION 0.8

mcp-echo-image:
    ARG BINARY_SOURCE=source
    ARG TAG=latest
    ARG GIT_SHA=""
    ARG BOTWORK_VERSION=""
    ARG BOTWORK_GIT_SHA=""
    FROM DOCKERFILE --platform=linux/amd64 \
        --build-arg BINARY_SOURCE=$BINARY_SOURCE \
        --build-arg GIT_SHA=$GIT_SHA \
        --build-arg BOTWORK_VERSION=$BOTWORK_VERSION \
        --build-arg BOTWORK_GIT_SHA=$BOTWORK_GIT_SHA \
        -f echo/Dockerfile .
    SAVE IMAGE botwork/mcp-echo:local
    IF [ "$TAG" = "latest" ]
        SAVE IMAGE --push ghcr.io/botworkz/mcp/mcp-echo:latest
    ELSE
        SAVE IMAGE --push ghcr.io/botworkz/mcp/mcp-echo:${TAG}
        SAVE IMAGE --push ghcr.io/botworkz/mcp/mcp-echo:latest
    END

images:
    ARG GIT_SHA=""
    ARG BOTWORK_VERSION=""
    ARG BOTWORK_GIT_SHA=""
    BUILD +mcp-echo-image \
        --GIT_SHA="$GIT_SHA" \
        --BOTWORK_VERSION="$BOTWORK_VERSION" \
        --BOTWORK_GIT_SHA="$BOTWORK_GIT_SHA"
