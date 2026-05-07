CONTAINER_RUNTIME ?= $(shell command -v podman 2>/dev/null || command -v docker 2>/dev/null)
IMAGE := mega-blastoise-test

.PHONY: build run

build:
	$(CONTAINER_RUNTIME) build -t $(IMAGE) .

# Always rebuild before running so the container reflects the latest source.
run: build
	$(CONTAINER_RUNTIME) run -i --rm $(IMAGE)
