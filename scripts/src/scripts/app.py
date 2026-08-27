import typer
from vespa.application import Vespa
from vespa.deployment import VespaDocker
from vespa.package import sample_package

from scripts import eprint

DEFAULT_CONTAINER_NAME = "sample"

main = typer.Typer()


@main.command()
def load(
    name: str = DEFAULT_CONTAINER_NAME,
    restart: bool = False,
) -> Vespa:
    try:
        eprint(f"> Loading {name} Vespa container")
        container = VespaDocker.from_container_name_or_id(name)

        if restart and container.container:
            container.container.restart()
            container.start_services()

        return Vespa(
            url=container.url,
            port=container.local_port,
            application_package=sample_package,
        )
    except ValueError:
        eprint(f"> Vespa container {name} not found, creating new")
        container = VespaDocker()
        if name != DEFAULT_CONTAINER_NAME and container.container:
            # TODO: check if this works
            container.container.rename(name)
        return container.deploy(sample_package)


if __name__ == "__main__":
    main()
