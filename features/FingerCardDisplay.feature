Feature: Finger card display
  As the single user of the finger server
  I want my card to look exactly like finger(1) long output
  So that any finger client reads it without surprise

  Background:
    Given the finger server is running on an ephemeral port
    And the user has a project file with content "Deploying thimbl"
    And the user has a plan file with content "Ship the finger server"

  Scenario: Long format mirrors finger(1) fields
    When a client connects and sends the user's login name
    Then the response contains a "Login name:" line with the login
    And the response contains an "In real life:" line with the real name
    And the response contains a "Directory:" line with the home directory
    And the response contains a "Shell:" line with the shell
    And the response contains a "Project:" section with "Deploying thimbl"
    And the response contains a "Plan:" section with "Ship the finger server"

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
