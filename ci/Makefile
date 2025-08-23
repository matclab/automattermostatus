
SHELL=bash

.DEFAULT_GOAL := help
.PHONY: help

help: ### Print this help message
	@grep -h -E '^[^ :]+:.*?## .*$$' $(MAKEFILE_LIST) \
	| sed -n 's/^\(.*\):\(.*\)##\(.*\)/\1▅\3/p' \
	| column -t  -s '▅'


docker: ### Build and push docker image with tag VERSION
	echo "${VERSION}"
	docker build . -t registry.gitlab.com/matclab/automattermostatus/buildandtest:${VERSION}
	docker tag registry.gitlab.com/matclab/automattermostatus/buildandtest:${VERSION} registry.gitlab.com/matclab/automattermostatus/buildandtest:latest
	docker build . -f Dockerfile.package -t registry.gitlab.com/matclab/automattermostatus/package:${VERSION}
	docker build . -f Dockerfile.wine -t registry.gitlab.com/matclab/automattermostatus/buildandtestwin:${VERSION}
	${MAKE} push VERSION=${VERSION}
push: ### Push docker package with VERSION
	echo "${VERSION}"
	docker push registry.gitlab.com/matclab/automattermostatus/buildandtest:${VERSION}
	docker push registry.gitlab.com/matclab/automattermostatus/package:${VERSION}
	docker push registry.gitlab.com/matclab/automattermostatus/buildandtestwin:${VERSION}
