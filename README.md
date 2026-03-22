# avatar-server

A simple alternative avatar server implementation for tangled based on
[core/avatar](https://tangled.org/tangled.org/core/tree/master/avatar)

To self-host you can use one of:
- the NixOS module
- the binary

The service makes a bunch of assumptions. For example:
- crtime being readable
- permissions set correctly on the cache dir
- `_atproto` TXT record is preferred over `.well-known/atproto-did`
- and probably others too

## Using the NixOS module

A simple configuration could look like the following:
```nix
{
    inputs,
    ...
}:

{
    imports = [ inputs.avatar-server.nixosModules.default ];

    services.tangled.avatar-server = {
        enable = true;
        port = 1234;
        # The environment file MUST be outside the nix store
        # and contain the AVATAR_SHARED_SECRET variable
        environmentFile = "/path/to/your/secret.env";
    };
}
```

## Using the binary

The following should suffice:
```sh
AVATAR_SHARED_SECRET=super_secret_value CACHE_DIR=/var/log/avatar-server avatar-server
```

I recommend using systemd to run this application in a container.
Other solutions like docker/podman probably work too, but i don't have any
experience with those services.

## Security

I honestly don't know how secure this program is but i tried my best
to harden it in systemd. Pull requests are very welcome! This is
one of my first Rust projects, so feel free to teach me
