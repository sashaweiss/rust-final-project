### API Ideas
As a client:
* Send a single pre-scripted command, get the output back in a useful format
* Read a line of stdin, send as a command, get output back
* Send a pre-scripted chat
* Read stdin as chat
* Connect and receive a struct with connectivity information
* Send the contents of a file as a series of commands

* Use futures

* Split stream instead of cloning


## Todo - 5/29
* Design an API
* Implement chat as a feature
* Line editing
* Configuration options (choose shell) via builder
* Ability to send editable lines to other clients

## Ideas
- Quick-start function that captures all stdin and replicates all stdout, in one easy line
- Send individual commands/arguments or do it in a stream. Use cases:
  -- Have a bot scripted to do `git` help
  -- Have a gui through which you can send commands
- Send chat commands that get colored, so you can chat and run stuff in the same place
- Have stuff get typed with a delay, so it looks more like typing
- Optionally send key-by-key stdin or on return
