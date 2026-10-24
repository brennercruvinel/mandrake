.PHONY: build install uninstall test manual cross clean

GOBIN_DIR := $(or $(shell go env GOBIN),$(shell go env GOPATH)/bin)

build:
	go build -o ../bin/ ./cmd/fancy-tint

# Installs the binary, then registers the statusline + hooks in ~/.claude/settings.json.
install:
	go install ./cmd/fancy-tint
	$(GOBIN_DIR)/fancy-tint install $(ARGS)

uninstall:
	$(GOBIN_DIR)/fancy-tint uninstall $(ARGS)
	rm -f $(GOBIN_DIR)/fancy-tint

test:
	go vet ./...
	go test ./...

# Checks what reaches the terminal; see ../scripts/manual-test.sh. Pass e.g. MANUAL=tmux.
manual:
	../scripts/manual-test.sh $(MANUAL)

cross:
	GOOS=linux   GOARCH=amd64 go build -o dist/fancy-tint-linux-amd64       ./cmd/fancy-tint
	GOOS=darwin  GOARCH=arm64 go build -o dist/fancy-tint-darwin-arm64      ./cmd/fancy-tint
	GOOS=darwin  GOARCH=amd64 go build -o dist/fancy-tint-darwin-amd64      ./cmd/fancy-tint
	GOOS=windows GOARCH=amd64 go build -o dist/fancy-tint-windows-amd64.exe ./cmd/fancy-tint

clean:
	rm -rf ../bin dist
