using InvoiceDesk.Services;
using Xunit;

public class TotalsTests
{
    [Fact]
    public void AddsVat() => Assert.Equal(108.10m, Totals.WithVat(100m, 0.081m));
}
