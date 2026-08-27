from vespa.package import sample_package
from vespa.application import Vespa
from vespa.deployment import VespaDocker


DEFAULT_CONTAINER_NAME = "sample"


def load(name: str = DEFAULT_CONTAINER_NAME) -> Vespa:
    try:
        print(f"> Loading {name} Vespa container")
        container = VespaDocker.from_container_name_or_id(name)
    except ValueError:
        print(f"> Vespa container {name} not found, creating new")
        container = VespaDocker()
        if name != DEFAULT_CONTAINER_NAME and container.container:
            container.container.rename(name)

    return container.deploy(sample_package)
