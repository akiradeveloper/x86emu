docker-build:
	docker build -t akiradeveloper:x86emu --build-arg USER=${USER} --build-arg UID=`id -u` - < Dockerfile