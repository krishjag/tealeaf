namespace TeaLeaf;

/// <summary>
/// Exception thrown when a TeaLeaf operation fails.
/// </summary>
public class TLException : Exception
{
    public TLException() : base() { }

    public TLException(string message) : base(message) { }

    public TLException(string message, Exception innerException)
        : base(message, innerException) { }
}
