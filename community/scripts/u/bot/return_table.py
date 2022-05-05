import numpy as np
import pandas as pd

def main(
    seed = 123
):
    np.random.seed(seed)
    df = pd.DataFrame(np.random.randint(0,100,size=(100, 4)), columns=list('ABCD'))
    print(df)
    print("See the result tab to see it rendered as a table")
    return [df.columns.values.tolist()] + df.values.tolist()
