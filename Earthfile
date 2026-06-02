VERSION 0.8

mcp-echo-image:
    FROM DOCKERFILE --platform=linux/amd64 -f containers/mcp-echo/Dockerfile .
    SAVE IMAGE botwork/mcp-echo:local

images:
    BUILD +mcp-echo-image
