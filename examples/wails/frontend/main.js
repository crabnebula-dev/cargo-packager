import { Greet } from "/wailsjs/go/main/App.js";

let nameElement = document.getElementById("name");
nameElement.focus();
let resultElement = document.getElementById("result");

// Setup the greet function
window.greet = () => {
  // Get name
  let name = nameElement.value;

  // Check if the input is empty
  if (name === "") return;

  // Call App.Greet(name)
  try {
    Greet(name)
      .then((result) => {
        // Update result with data back from App.Greet()
        resultElement.innerText = result;
      })
      .catch((err) => {
        console.error(err);
      });
  } catch (err) {
    console.error(err);
  }
};
