CARGO ?= cargo

# Targets
WINDOWS_x64 = x86_64-pc-windows-msvc
WINDOWS_aarch64 = aarch64-pc-windows-msvc
LINUX_x64 = x86_64-unknown-linux-gnu
LINUX_aarch64 = aarch64-unknown-linux-gnu
LINUX_x64_musl = x86_64-unknown-linux-musl
LINUX_aarch64_musl = aarch64-unknown-linux-musl

all: release

.PHONY: release
release:
	$(CARGO) build --release

.PHONY: debug
debug:
	$(CARGO) build

.PHONY: cross-compile
cross-compile: win-x64 \
  linux-x64 linux-x64-musl \
  linux-aarch64 linux-aarch64-musl

.PHONY: win-x64
win-x64:
	$(CARGO) build --release --target=$(WINDOWS_x64)

# Not supported by rquickjs
.PHONY: win-aarch64
win-aarch64:
	$(CARGO) build --release --target=$(WINDOWS_aarch64)

.PHONY: print-win-aarch64-triple
print-win-x64-triple:
	@echo $(WINDOWS_aarch64)

.PHONY: linux-x64
linux-x64:
	$(CARGO) build --release --target=$(LINUX_x64)

.PHONY: print-linux-x64-triple
print-linux-x64-triple:
	@echo $(LINUX_x64)

.PHONY: linux-aarch64
linux-aarch64:
	$(CARGO) build --release --target=$(LINUX_aarch64)

.PHONY: print-linux-aarch64-triple
print-linux-aarch64-triple:
	@echo $(LINUX_aarch64)

.PHONY: linux-x64-musl
linux-x64-musl:
	$(CARGO) build --release --target=$(LINUX_x64_musl)

.PHONY: print-linux-x64-musl-triple
print-linux-x64-musl-triple:
	@echo $(LINUX_x64_musl)

.PHONY: linux-aarch64-musl
linux-aarch64-musl:
	$(CARGO) build --release --target=$(LINUX_aarch64_musl)

.PHONY: print-linux-aarch64-musl-triple
print-linux-aarch64-musl-triple:
	@echo $(LINUX_aarch64_musl)


.PHONY: clean
clean:
	$(CARGO) clean
