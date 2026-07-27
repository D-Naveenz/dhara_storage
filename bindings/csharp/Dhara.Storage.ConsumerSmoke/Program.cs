using Dhara.Storage;

var root = Path.Combine(Path.GetTempPath(), "dhara-storage-consumer-smoke", Guid.NewGuid().ToString("N"));
Directory.CreateDirectory(root);

try
{
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
    if (Directory.Exists(root))
    {
        Directory.Delete(root, recursive: true);
    }
}
