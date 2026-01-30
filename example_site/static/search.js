// 1. Initialize MiniSearch
let miniSearch = new MiniSearch({
  fields: ["title", "content"], 
  storeFields: ["title", "url", "collection"], 
  searchOptions: {
    boost: { title: 2 },
    fuzzy: 0.2,
    prefix: true 
  },
});

// 2. Fetch the Index
fetch("/search.json")
  .then((response) => response.json())
  .then((allPosts) => {
    miniSearch.addAll(allPosts);
  });

// 3. UI & Search Logic
document.addEventListener("DOMContentLoaded", () => {
  const toggleBtn = document.getElementById("search-toggle");
  const wrapper = document.getElementById("search-wrapper");
  const input = document.getElementById("search-input");
  const resultsContainer = document.getElementById("search-results");

  // Toggle Logic (Same as before)
  toggleBtn.addEventListener("click", () => {
    const isHidden = wrapper.classList.contains("hidden");
    if (isHidden) {
      wrapper.classList.remove("hidden");
      setTimeout(() => input.focus(), 50);
    } else {
      wrapper.classList.add("hidden");
      input.blur();
    }
  });

  // Close behaviors (Same as before)
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && !wrapper.classList.contains("hidden")) {
      wrapper.classList.add("hidden");
      input.blur();
    }
  });

  document.addEventListener("click", (e) => {
    const isClickInside = wrapper.contains(e.target) || toggleBtn.contains(e.target);
    const isHeader = e.target.closest(".header-inner");
    if (!isClickInside && !isHeader && !wrapper.classList.contains("hidden")) {
      wrapper.classList.add("hidden");
    }
  });

  // --- UPDATED SEARCH LOGIC ---
  input.addEventListener("input", (e) => {
    const query = e.target.value;

    // Only search if user typed at least 2 characters (Reduces noise)
    if (query.length > 1) {
      const results = miniSearch.search(query);
      renderResults(results);
    } else {
      // If empty or just 1 char, clear results but DON'T show "No results"
      resultsContainer.innerHTML = "";
    }
  });

  function renderResults(results) {
    if (results.length === 0) {
      resultsContainer.innerHTML =
        '<div class="result-item"><p style="color: #666; font-style: italic;">No results found.</p></div>';
      return;
    }

    resultsContainer.innerHTML = results
      .map(
        (result) => `
            <div class="result-item">
                <a href="${result.url}">
                    <h3>${result.title}</h3>
                    <span class="badge">${result.collection}</span>
                    </a>
            </div>
        `
      )
      .join("");
  }
});