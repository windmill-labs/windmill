import seaborn as sns
import base64
import numpy as np
import pandas as pd

def main(
    seed: int = 1234
):
    np.random.seed(seed)
    data = np.random.multivariate_normal([0, 0], [[5, 2], [2, 2]], size=2000)
    data = pd.DataFrame(data, columns=['x', 'y'])

    file_output = "output.png"

    with sns.axes_style('white'):
        plot = sns.jointplot("x", "y", data, kind='hex')
        fig = plot.figure.savefig(file_output)

    with open(file_output, "rb") as image_file:
        encoded_string = base64.b64encode(image_file.read()).decode('ascii')

    return {"png": encoded_string}