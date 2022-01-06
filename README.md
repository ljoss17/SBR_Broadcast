# SBR_Broadcast

## Configuration

The configuration of the Broadcast is in the `broadcast.config`. All fields are optional, with a default value if not specified :

* addr : The address of the Rendezvous server. Default : 127.0.0.1
* port : The port of the Rendezvous server. Default : 4446
* spawn : How many processes to spawn. Default : 100
* N : The size of the entire system. Default : 100
* G : The size of the Gossip peers set. Default : 10
* E : The size of the Echo peers set. Default : 40
* E_thr : The Echo threshold. Default : 10
* R : The size of the Ready peers set. Default : 30
* R_thr : The Ready threshold. Default : 10
* D : The size of the Delivery peers set. Default : 25
* D_thr : The Delivery threshold. Default : 14

## Commands

The Rendezvous server and the Broadcast both accept input commands. The following commands exist for the Rendezvous server :

* exit : Stops the server

The following commands exist for the Broadcast :

* send : Trigger the signal to a random process to dispatch a Message
* exit : Stops the Broadcast

## Running

Before running the Broadcast, run the Rendezvous server with the following command :

```
cargo run --bin rendezvous
```

Once the Rendezvous server is running, the Broadcast can be started with :

```
cargo run
```

To initiate the dispatch of a Message by a random process, run the command :

```
send
```

## Documentation

The documentation for the code can be found [here](https://ljoss17.github.io/SBR_Broadcast/sbr_broadcast/).
