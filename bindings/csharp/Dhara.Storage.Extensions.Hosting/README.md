# Dhara.Storage.Extensions.Hosting

[![NuGet](https://img.shields.io/nuget/v/Dhara.Storage.Extensions.Hosting)](https://www.nuget.org/packages/Dhara.Storage.Extensions.Hosting)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/D-Naveenz/dhara_storage/blob/main/LICENSE.txt)

.NET Generic Host integration for [Dhara.Storage](https://www.nuget.org/packages/Dhara.Storage). Registers a hosted service that starts the `dhara-sd` sidecar with your application and stops it on shutdown, so you do not need to manage the daemon lifetime yourself.

## Install

```powershell
dotnet add package Dhara.Storage.Extensions.Hosting --version 0.9.6
```

## Usage

```csharp
using Dhara.Storage.Extensions.Hosting;

var builder = Host.CreateApplicationBuilder(args);
builder.Services.AddDharaStorage();

var app = builder.Build();
await app.RunAsync();
```

Pass a configuration delegate to `AddDharaStorage` when you need a specific named pipe or an
explicit `dhara-sd` executable path (for example, in a packaged deployment):

```csharp
builder.Services.AddDharaStorage(options =>
{
    options.PipeName = "my-app-dhara-sd";
});
```

## Related

- Product overview: [Dhara Storage][root]
- Storage API: [Dhara.Storage][storage]

## License

Apache-2.0.

[root]: https://github.com/D-Naveenz/dhara_storage
[storage]: https://www.nuget.org/packages/Dhara.Storage
