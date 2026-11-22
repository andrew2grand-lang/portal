using System.Windows.Controls;

namespace InvoiceDesk.Services;

public static class Printing
{
    public static void PrintSelected(DataGrid grid)
    {
        var dialog = new PrintDialog();
        if (dialog.ShowDialog() == true) dialog.PrintVisual(grid, "Invoices");
    }
}
