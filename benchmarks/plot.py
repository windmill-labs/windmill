import matplotlib.pyplot as plt
import json

with open("/tmp/windmill/profiling.json") as json_file:
    data = json.load(json_file)
    i = 0
    for impacts in data["timings"][:]:
        i += 1
        if i % 200 != 0:
            continue
        timefilteredForce = plt.plot(impacts)
        plt.text(len(impacts) - 1, impacts[-1], f"{i}")
        timefilteredForce = plt.xlabel("points")
        timefilteredForce = plt.ylabel("Force fast")

    # for impacts in data["timings"][0:100]:
    #     timefilteredForce = plt.plot(impacts)
    #     timefilteredForce = plt.xlabel("points")
    #     timefilteredForce = plt.ylabel("Force slow")

    plt.show()
