namespace TeaLeaf;

/// <summary>
/// Exception thrown when a TeaLeaf operation fails.
/// </summary>
public class TLException : Exception
{
    /// <summary>Initializes a new instance of <see cref="TLException"/>.</summary>
    public TLException() : base() { }

    /// <summary>Initializes a new instance of <see cref="TLException"/> with a message.</summary>
    /// <param name="message">The error message.</param>
    public TLException(string message) : base(message) { }

    /// <summary>Initializes a new instance of <see cref="TLException"/> with a message and inner exception.</summary>
    /// <param name="message">The error message.</param>
    /// <param name="innerException">The exception that caused this error.</param>
    public TLException(string message, Exception innerException)
        : base(message, innerException) { }
}
