import { test, expect } from "@playwright/test";

test.describe("SME workflow", () => {
  test("nav bar renders + browse page loads", async ({ page }) => {
    await page.goto("/");
    // Redirects to /browse
    await expect(page).toHaveURL(/\/browse/);
    // Header has "kg editor" brand + 3 nav links
    await expect(page.getByText("kg editor")).toBeVisible();
    await expect(page.getByRole("link", { name: "Browse" })).toBeVisible();
    await expect(page.getByRole("link", { name: "Graph" })).toBeVisible();
    await expect(page.getByRole("link", { name: "Search" })).toBeVisible();
  });

  test("search page input is reachable", async ({ page }) => {
    await page.goto("/search");
    const input = page.getByPlaceholder("Search across all entities…");
    await expect(input).toBeVisible();
  });
});
