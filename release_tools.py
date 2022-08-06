import argparse
import json
import urllib.request
from subprocess import check_output

def main() -> None:
    parser = argparse.ArgumentParser(description="Some small utility to create releases")
    parser.add_argument("task", type=str, default="version", choices=["version", "check_tag"])
    args = parser.parse_args()

    if args.task == "version":
        print(get_version())
    elif args.task == "check_tag":
        check_tag()
    else:
        raise Exception("Unknown option")

def check_tag() -> None:
    """
    This is a very naive way of checkin if we already have a release with that
    tag. We want to avoid overwriting already existing releases, so this
    function will raise an exception when urllib does a successful request or
    when it receives an error differently to a 404.

    In this case a 404 is what we expect.
    """
    base_url = "https://github.com/etra0/litcher/releases/tag/{}"
    current_ver = get_version()
    final_url = base_url.format(current_ver)
    try:
        with urllib.request.urlopen(final_url) as req:
            if req.status == 200:
                raise Exception("There's already a tag {}".format(current_ver))
    except urllib.error.HTTPError as e:
        if e.status != 404:
            raise e

    print("No release was detected with the tag", current_ver)
    return


def get_version() -> None:
    manifest = json.loads(check_output(["cargo", "read-manifest", "--manifest-path", "Cargo.toml"]))
    return "v{}".format(manifest['version'])

if __name__ == "__main__":
    main()
