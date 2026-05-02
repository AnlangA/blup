import os

# Fork repeatedly — should hit pids-limit before host is affected
for _ in range(50):
    try:
        os.fork()
    except OSError:
        print("PROCESS_LIMIT_HIT")
        break
