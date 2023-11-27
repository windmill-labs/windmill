import functools
import ast
import inspect
from graphlib import TopologicalSorter

def flow(f):
    @functools.wraps(f)
    def wrapper(*args, **kwargs):
        
        return f(*args, **kwargs)

    return wrapper

class Edge(object):
    def __init__(self, _node, _path):
        self._node = _node
        self._path = _path
        print("DECL EDGE: node: " + str(self._node) + " path: " + str(self._path))

    def __getattr__(self, attr):
       return  super(self._node, self._path + [attr])

    def __str__(self):
        return f"EDGE: {self._node}({self._path})"

class Node(object):
    def __init__(self, _id, _deps):
        self._id = _id
        self._deps = _deps
        print("DECL NODE: id: " + str(self._id) + ", deps: " + str([f"{k}={v.__str__()}" for k, v in self._deps.items()]))


    def __getattr__(self, attr):
       return  Edge(self, [attr])

    def __str__(self):
        k_v = [f"{k}={v.__str__()}" for k, v in self._deps.items()]
        return f"{self._id}({k_v})"

def step(f):
    print("DECL F: " + f.__name__)
    # code = inspect.getsource(f)
    # print(code)
    # print(ast.dump(ast.parse(code)))
    # print(ast.unparse(ast.parse(code)))
    @functools.wraps(f)
    def wrapper(*args, **kwargs):
        kwargs.update(dict(zip(f.__code__.co_varnames, args)))
        for k, v in kwargs.items():
            if isinstance(v, Node):
                kwargs[k] = Edge(v, [])
        return Node(f.__name__, kwargs)

    return wrapper

@step
def a(a):
    import pandas
    return 32

@step
def b(b):
    import pandas
    return 32

@step
def c(x, y):
    return x + y + 1

def flow():
    r_a = a(1)
    r_b = b(2)
    r_c = c(r_a, r_b)
    return r_c

def get_edges(graph):
    edges = {}
    nodes = [graph]
    seen = set()
    for k, v in graph._deps.items():
        if isinstance(v, Edge):
            edges.append((v._node._id, v._id))
            nodes.append(v._node)
    return edges

def render(graph):
    print(graph)
    edges = get_edges(graph)
    ts = TopologicalSorter(edges)
    o = tuple(ts.static_order())
    print(o)

render(flow())