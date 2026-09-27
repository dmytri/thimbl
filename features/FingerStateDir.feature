Feature: Finger state directory
  As the single user of the finger server
  I want my card files to live in one state directory
  So that the server serves exactly what I publish and reads nothing else from my home

  The state directory is the fixed path $HOME/.local/share/thimbl.

  Scenario: An empty state directory serves the default card
    Given the state directory has no project file
    And the state directory has no plan file
    And the finger server is running on an ephemeral port
    When a client connects and sends the user's login name
    Then the response contains a "Project:" section with "No Project."
    And the response contains a "Plan:" section with "No Plan."

  Scenario: The first run seeds the state directory from the home dot-files
    Given the home has a project file with content "Deploying thimbl"
    And the home has a plan file with content "Ship the finger server"
    And the finger server is running on an ephemeral port
    Then the state directory has the project content "Deploying thimbl"
    And the state directory has the plan content "Ship the finger server"
    When a client connects and sends the user's login name
    Then the response contains a "Project:" section with "Deploying thimbl"
    And the response contains a "Plan:" section with "Ship the finger server"

  Scenario: An existing state directory is not overwritten by the home dot-files
    Given the state directory has a plan file with content "Already published"
    And the home has a plan file with content "Stale draft"
    And the finger server is running on an ephemeral port
    When a client connects and sends the user's login name
    Then the response contains a "Plan:" section with "Already published"
    And the response does not contain "Stale draft"

  Scenario: Plan edits in the state directory are served live
    Given the state directory has a plan file with content "Working draft"
    And the finger server is running on an ephemeral port
    When the state directory plan is changed to "Updated plan"
    And a client connects and sends the user's login name
    Then the response contains a "Plan:" section with "Updated plan"

  Scenario: The card files live at the fixed state directory path
    Given the finger server is running on an ephemeral port
    When a client connects and sends the user's login name
    Then "$HOME/.local/share/thimbl" contains the card files
