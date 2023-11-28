import functools
import ast
import inspect
import yaml
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
        # print("DECL EDGE: node: " + str(self._node) + " path: " + str(self._path))

    def __getattr__(self, attr):
       return Edge(self._node, self._path + [attr])

    def __str__(self):
        return f"EDGE: {self._node}({self._path})"

class Node(object):
    def __init__(self, _id, _deps, _f):
        self._id = _id
        self._deps = _deps
        self._f = _f
        # print("DECL NODE: id: " + str(self._id) + ", deps: " + str([f"{k}={v.__str__()}" for k, v in self._deps.items()]))


    def __getattr__(self, attr):
       return  Edge(self, [attr])

    def __str__(self):
        k_v = [f"{k}={v.__str__()}" for k, v in self._deps.items()]
        return f"{self._id}({k_v})"

def step(f):
    # print("DECL F: " + f.__name__)
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
        return Node(f.__name__, kwargs, f)

    return wrapper

@step
def d():
    return 32

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
    r_a = a(d())
    r_b = b(2)
    r_c = c(r_a, r_b)
    return r_c

def get_edges_and_nodes(graph):
    edges = {}
    nodes = [graph]
    all_nodes = []
    seen = set()
    while nodes:
        node = nodes.pop()
        id = node._id
        if id in seen:
            continue
        all_nodes.append(node)
        seen.add(id)
        for _, v in node._deps.items():
            if isinstance(v, Edge):
                if id not in edges:
                    edges[id] = []
                edges[id].append(v._node._id)
                nodes.append(v._node)
    return edges, {n._id: n for n in all_nodes}

def render(graph):
    edges, all_nodes = get_edges_and_nodes(graph)
    # print(edges)
    # print(all_nodes)
    ts = TopologicalSorter(edges)
    sortd = tuple(ts.static_order())
    modules_f = [[id, all_nodes[id]] for id in sortd]
    files = [ n._f for n in all_nodes.values()]
    sources = gen_sources(files)
    print(sources)
    modules = [{"id": k, "lang": "python", "content": render_f(v._f)} for k, v in modules_f]
    return yaml.dump(modules)

def gen_module_sources(source):
    pass

def gen_sources(files):
    files = set(files)
    asts = []
    sources = {}
    for f in files:
        asts.append(ast.parse(inspect.getsource(f)))
        
    for astf in asts:
        for node in astf.body:
            if isinstance(node, ast.FunctionDef):
                print(node.args)
                code =  ast.unparse(node.body)
                sources[node.name] = code

    return sources

def render_f(f):
    code = inspect.getsource(f)
    return ast.unparse(ast.parse(code))

rendered = render(flow())
