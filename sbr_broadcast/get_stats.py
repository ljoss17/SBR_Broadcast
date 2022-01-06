success = 0
acc_setup = 0
acc_send = 0
min_setup = 999
max_setup = 0
min_send = 999
max_send = 0
for i in range(100):
    f = open(f"results_{i}.log", "r")
    lines = f.readlines()
    if lines[0] == "SUCCESS\n":
        success += 1
        setup = int(lines[1].split(" : ")[1].split(" ")[0])
        send = int(lines[2].split(" : ")[1].split(" ")[0])
        acc_setup += setup
        acc_send += send
        if setup > max_setup:
            max_setup = setup
        if setup < min_setup:
            min_setup = setup

        if send > max_send:
            max_send = send
        if send < min_send:
            min_send = send
print(f"success : {success}")
print(f"Mean setup : {acc_setup/success}")
print(f"Mean send : {acc_send/success}")
print(f"Min setup : {min_setup}")
print(f"Max setup : {max_setup}")
print(f"Min send : {min_send}")
print(f"Max send : {max_send}")
