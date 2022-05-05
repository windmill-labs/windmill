import wmill

import matplotlib.pyplot as plt
import base64
from datetime import date


client = wmill.Client()


def main(rows: list, title: str = ""):
    file_output = "output.png"

    fig, ax = plt.subplots(figsize=(6, 3), subplot_kw=dict(aspect="equal"))

    # Get labels and data from list
    labels = [x[0] for x in rows]
    data = [x[1] for x in rows]

    # Create donut chart
    plt.pie(
        data,
        labels=labels,
        autopct="%.0f%%",
        wedgeprops={"linewidth": 5, "edgecolor": "white"},
    )
    my_circle = plt.Circle((0, 0), 0.38, color="white")
    p = plt.gcf()
    p.gca().add_artist(my_circle)
    ax.set_title(title)

    # Create png
    plt.savefig(file_output)

    with open(file_output, "rb") as image_file:
        encoded_string = base64.b64encode(image_file.read()).decode("ascii")

    return {"png": encoded_string}
