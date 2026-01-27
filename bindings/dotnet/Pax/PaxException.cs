namespace Pax;

/// <summary>
/// Exception thrown when a Pax operation fails.
/// </summary>
public class PaxException : Exception
{
    public PaxException() : base() { }

    public PaxException(string message) : base(message) { }

    public PaxException(string message, Exception innerException)
        : base(message, innerException) { }
}
