using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.Json.Serialization.Metadata;

namespace Dhara.Storage.Interop.Native;

internal static class NativeJson
{
    internal static T Deserialize<T>(string json)
    {
        var typeInfo = GetTypeInfo<T>();
        return JsonSerializer.Deserialize(json, typeInfo)
            ?? throw new InvalidOperationException("Native JSON payload could not be deserialized.");
    }

    private static JsonTypeInfo<T> GetTypeInfo<T>() => (JsonTypeInfo<T>)(object)(typeof(T) switch
    {
        var type when type == typeof(NativeErrorPayload) => NativeJsonContext.Default.NativeErrorPayload,
        var type when type == typeof(NativeAnalysisReportDto) => NativeJsonContext.Default.NativeAnalysisReportDto,
        var type when type == typeof(NativeFileInformationDto) => NativeJsonContext.Default.NativeFileInformationDto,
        var type when type == typeof(NativeDirectoryInformationDto) => NativeJsonContext.Default.NativeDirectoryInformationDto,
        var type when type == typeof(NativeStorageEntryDto[]) => NativeJsonContext.Default.NativeStorageEntryDtoArray,
        var type when type == typeof(NativeWatchEventDto) => NativeJsonContext.Default.NativeWatchEventDto,
        var type when type == typeof(NativeLogRecordDto) => NativeJsonContext.Default.NativeLogRecordDto,
        _ => throw new NotSupportedException($"No generated JSON metadata is available for '{typeof(T).FullName}'."),
    });
}

[JsonSourceGenerationOptions(
    PropertyNameCaseInsensitive = true,
    PropertyNamingPolicy = JsonKnownNamingPolicy.SnakeCaseLower,
    GenerationMode = JsonSourceGenerationMode.Metadata)]
[JsonSerializable(typeof(NativeErrorPayload))]
[JsonSerializable(typeof(NativeAnalysisReportDto))]
[JsonSerializable(typeof(NativeDetectedDefinitionDto))]
[JsonSerializable(typeof(NativeFileInformationDto))]
[JsonSerializable(typeof(NativeDirectoryInformationDto))]
[JsonSerializable(typeof(NativeDirectorySummaryDto))]
[JsonSerializable(typeof(NativeStorageMetadataDto))]
[JsonSerializable(typeof(NativeStorageEntryDto[]), TypeInfoPropertyName = "NativeStorageEntryDtoArray")]
[JsonSerializable(typeof(NativeWatchEventDto))]
[JsonSerializable(typeof(NativeLogRecordDto))]
internal sealed partial class NativeJsonContext : JsonSerializerContext
{
}
