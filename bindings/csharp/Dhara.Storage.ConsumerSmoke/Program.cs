using Dhara.Storage;
using Dhara.Storage.Extensions.Hosting;
using Microsoft.Extensions.Hosting;

var root = Path.Combine(Path.GetTempPath(), "dhara-storage-consumer-smoke", Guid.NewGuid().ToString("N"));
Directory.CreateDirectory(root);

var builder = Host.CreateApplicationBuilder(args);
builder.Services.AddDharaStorage();
var host = builder.Build();

try
{
    await host.StartAsync().ConfigureAwait(false);

    var filePath = Path.Combine(root, "sample.txt");
    var storageFile = DharaStorage.File(filePath);

    storageFile.WriteText("native aot check");
    var text = storageFile.ReadText();
    var info = storageFile.RefreshInformation();

    Console.WriteLine($"{Path.GetFileName(filePath)}|{text}|{info.Size}");
    return 0;
}
finally
{
    await host.StopAsync().ConfigureAwait(false);

    if (Directory.Exists(root))
    {
        Directory.Delete(root, recursive: true);
    }
}
