initSidebarItems({"fn":[["check_echoes","Check the status of the Echo replies received. If more than the threshold have been received, Probabilistic Consistent Broadcast deliver the Message."],["deliver","Probabilistic Broadcast Deliver. If the Message is verified, send an Echo of the Message to the Echo peers."],["deliver_echo","Deliver an Echo type Message. Save the Echo Message in the Echo replies, used to track when a Message is ready to be delivered."],["echo_subscribe","Send EchoSubscription to Echo peers."],["echo_subscription","Deliver an EchoSubscription type Message. If an Echo Message has already been delivered, send it to the subscribing Node."],["init","Initialises the Echo set used in the Sieve algorithm. Sample randomly a number of peers from the system and send them an EchoSubscription."]]});