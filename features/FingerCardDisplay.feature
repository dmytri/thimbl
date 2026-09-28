Feature: Finger card display
  As the single user of the finger server
  I want my card to carry one identity line and my published text
  So that any finger client reads it without surprise

  The identity line resolves in this order: the user file in the state
  directory, then the passwd comment field, then the login. An empty
  value falls through to the next source.

  Background:
    Given the finger server is running on an ephemeral port
    And the user has a project file with content "Deploying thimbl"
    And the user has a plan file with content "Ship the finger server"

  Scenario: The card carries one identity line from the passwd comment
    When a client connects and sends the user's login name
    Then the response contains a "User:" line with the comment field
    And the response does not contain "Login name:"
    And the response does not contain "In real life:"
    And the response does not contain "Directory:"
    And the response does not contain "Shell:"
    And the response contains a "Project:" section with "Deploying thimbl"
    And the response contains a "Plan:" section with "Ship the finger server"

  @empty-comment-fixture
  Scenario: An empty comment field falls back to the login
    Given the user's passwd comment is empty
    When a client connects and sends the user's login name
    Then the response contains a "User:" line with the login

  Scenario: The user file supplies the identity line
    Given the state directory has a user file with content "Captain Dmytri"
    And the finger server is running on an ephemeral port
    When a client connects and sends the user's login name
    Then the response contains a "User:" line with "Captain Dmytri"

  Scenario: Identity edits in the user file are served live
    Given the state directory has a user file with content "Captain Dmytri"
    And the finger server is running on an ephemeral port
    When the state directory user is changed to "Boatswain"
    And a client connects and sends the user's login name
    Then the response contains a "User:" line with "Boatswain"

  Scenario: An empty user file falls through to the comment
    Given the state directory has a user file with content ""
    And the finger server is running on an ephemeral port
    When a client connects and sends the user's login name
    Then the response contains a "User:" line with the comment field

  @empty-comment-fixture
  Scenario: With an empty comment an empty user file falls back to the login
    Given the user's passwd comment is empty
    And the state directory has a user file with content ""
    And the finger server is running on an ephemeral port
    When a client connects and sends the user's login name
    Then the response contains a "User:" line with the login

  Scenario: Lines end with CRLF
    When a client connects and sends the user's login name
    Then every line of the response ends with CRLF

  Scenario: Project and plan are read live on every query
    Given the plan file content is changed to "Updated plan"
    When a client connects and sends the user's login name
    Then the response contains a "Plan:" section with "Updated plan"
    And the response does not contain "Ship the finger server"

  Scenario: Missing plan file shows No Plan
    Given the user has no plan file
    When a client connects and sends the user's login name
    Then the response contains a "Plan:" section with "No Plan."

  Scenario: Missing project file shows No Project
    Given the user has no project file
    When a client connects and sends the user's login name
    Then the response contains a "Project:" section with "No Project."

  Scenario: Server prints the bound port at startup
    Given the finger server is started on port 0
    When the startup output is read
    Then it contains the bound address and port
