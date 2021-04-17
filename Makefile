ifeq ($(OS),Windows_NT)
	WSL:=wsl --
	USB:=F:/
	SET_ENV:=
else
	WSL:=
	USB:=/run/media/elekrisk/6D95-4DD4/
	SET_ENV:=HHH_MAX_SCREEN_SIZE=(1920,900)
endif

.PHONY: all, all, run, debug, install

all:
	python3 make.py build

run:
	python3 make.py run

debug:
	python3 make.py run debug

install:
	python3 make.py install