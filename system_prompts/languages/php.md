# PHP

## Structure

The script must start with `<?php` and contain at least one function called `main`:

```php
<?php

function main(string $param1, int $param2) {
    return ["result" => $param1, "count" => $param2];
}
```

## Resource Types

On Windmill, credentials and configuration are stored in resources and passed as parameters to main.

You need to **redefine** the type of the resources that are needed before the main function. Always check if the class already exists using `class_exists`:

```php
<?php

if (!class_exists('Postgresql')) {
    class Postgresql {
        public string $host;
        public int $port;
        public string $user;
        public string $password;
        public string $dbname;
    }
}

function main(Postgresql $db) {
    // $db contains the database connection details
}
```

The resource type name has to be exactly as specified.

## Library Dependencies

Specify library dependencies as comments before the main function:

```php
<?php

// require:
// guzzlehttp/guzzle
// stripe/stripe-php@^10.0

function main() {
    // Libraries are available
}
```

One dependency per line. No need to require autoload, it is already done.
