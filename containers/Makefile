SHELL := /bin/bash

.DEFAULT_GOAL := containers

EARTHLY ?= earthly
VERSION := $(shell [ -f ../VERSION ] && tr -d '[:space:]' < ../VERSION || echo 0.0.0-dev)
GHCR_PREFIX := ghcr.io/botworkz/mcp

SERVICES := mcp-echo

.PHONY: containers tarballs clean

containers:
	$(EARTHLY) +images
	@if [ -n "$(VERSION)" ]; then \
		for svc in $(SERVICES); do \
			docker tag botwork/$$svc:local $(GHCR_PREFIX)/$$svc:$(VERSION); \
			docker tag botwork/$$svc:local $(GHCR_PREFIX)/$$svc:latest; \
		done; \
	fi

tarballs: containers
	mkdir -p dist
	@for svc in $(SERVICES); do \
		docker save botwork/$$svc:local -o dist/$$svc.tar; \
	done

clean:
	rm -rf dist
