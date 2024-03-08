import os

import setuptools

module_path = os.path.join(os.path.dirname(__file__), "windmill_helloworld.py")

setuptools.setup(
    name="windmill-helloworld",
    version="0.0.1",
    url="https://github.com/windmill-labs/windmill/blob//examples/deploy/private-package-registry-tls/README.md",
    author="WindmillLabs",
    author_email="contact@windmill.dev",
    description="Simple hello world python module to host on a private Pypi server",
    long_description=open("README.md").read(),
    py_modules=["windmill_helloworld"],
    zip_safe=False,
    platforms="any",
    install_requires=[],
    classifiers=[
        "Development Status :: 2 - Pre-Alpha",
        "Environment :: Web Environment",
        "Intended Audience :: Developers",
        "Operating System :: OS Independent",
        "Programming Language :: Python",
        "Programming Language :: Python :: 2",
        "Programming Language :: Python :: 2.7",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.3",
    ],
)
