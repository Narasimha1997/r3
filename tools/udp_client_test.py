import socket
import sys


ip = sys.argv[1]
port = sys.argv[2]

test_sock = socket.socket(family=socket.AF_INET, type=socket.SOCK_DGRAM)
test_sock.bind(('0.0.0.0', 8000))

to_send = "hello, from ubuntu"

while True:

    data, addr = test_sock.recvfrom(1024)
    print(data.decode("utf-8"), addr)

    # send the packet
    test_sock.sendto(to_send.encode("utf-8"), (ip, int(port)))