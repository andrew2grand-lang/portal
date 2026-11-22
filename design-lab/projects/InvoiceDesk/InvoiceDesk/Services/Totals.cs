namespace InvoiceDesk.Services;

public static class Totals
{
    public static decimal WithVat(decimal net, decimal rate) => decimal.Round(net * (1 + rate), 2);
}
