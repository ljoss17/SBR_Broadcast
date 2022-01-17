from math import floor

success = 0
acc_setup = 0
acc_send = 0
min_setup = 999
max_setup = 0
min_send = 999
max_send = 0
setup_array = []
send_array = []
for i in range(100):
    f = open(f"results_{i}.log", "r")
    lines = f.readlines()
    if lines[0] == "SUCCESS\n":
        success += 1
        setup = int(lines[1].split(" : ")[1].split(" ")[0])
        send = int(lines[2].split(" : ")[1].split(" ")[0])
        setup_array.append(setup)
        send_array.append(send)
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
setup_array.sort()
send_array.sort()
print(f"success : {success}")
print(f"Mean setup : {acc_setup/success}")
print(f"Mean send : {acc_send/success}")
print(f"Min setup : {min_setup}")
print(f"Max setup : {max_setup}")
print(f"Min send : {min_send}")
print(f"Max send : {max_send}")
half = floor(len(setup_array)/2)
if len(setup_array)%2 == 1:
    setup_median = setup_array[half]
    send_median = send_array[half]
else:
    setup_median = (setup_array[half-1]+setup_array[half])/2
    send_median = (send_array[half-1]+send_array[half])/2
setup_percentile = setup_array[floor(len(setup_array)*0.75)]
send_percentile = send_array[floor(len(send_array)*0.75)]
setup_percentile_low = setup_array[floor(len(setup_array)*0.25)]
send_percentile_low = send_array[floor(len(send_array)*0.25)]
print(f"Median setup : {setup_median}")
print(f"Median send : {send_median}")
print(f"75th percentile setup : {setup_percentile}")
print(f"75th percentile send : {send_percentile}")
print(f"25th percentile setup : {setup_percentile_low}")
print(f"25th percentile send : {send_percentile_low}")
